// HeadlessCConn: FLTK-free CConnection subclass for the Rust bridge.
// Implements all CConnection callbacks by calling into Rust via cxx.

#include "headless_conn.h"

#include <stdexcept>
#include <string>

#include <core/LogWriter.h>
#include <rdr/FdInStream.h>
#include <rdr/FdOutStream.h>
#include <rfb/CMsgWriter.h>
#include <rfb/PixelBuffer.h>
#include <rfb/PixelFormat.h>
#include <rfb/ScreenSet.h>
#include <rfb/encodings.h>
#include <network/TcpSocket.h>

#include "tigervnc-plus/src/bridge.rs.h"

static core::LogWriter vlog("HeadlessCConn");

namespace vnc_bridge {

VncConnection::VncConnection()
    : sock_(nullptr),
      connected_(false),
      serverPort_(0),
      pixelBuffer_(nullptr),
      lastEncoding_(rfb::encodingTight)
{
    supportsLocalCursor = true;
    supportsCursorPosition = true;
    supportsDesktopResize = true;
    supportsLEDState = true;
}

VncConnection::~VncConnection()
{
    if (connected_)
        close();
    delete sock_;
    // pixelBuffer_ is owned by CConnection (via setFramebuffer) and
    // deleted by CConnection::close() -> setFramebuffer(nullptr).
}

bool VncConnection::connect(const std::string& host, int port)
{
    try {
        serverHost_ = host;
        serverPort_ = port;

        network::TcpSocket* tcpSock = new network::TcpSocket(host.c_str(), port);
        sock_ = tcpSock;

        setServerName(host.c_str());
        setStreams(&sock_->inStream(), &sock_->outStream());
        initialiseProtocol();
        connected_ = true;

        vlog.info("Connected to %s:%d", host.c_str(), port);
        return true;
    } catch (std::exception& e) {
        std::string msg = "Connection to " + host + ":" + std::to_string(port)
                          + " failed: " + e.what();
        vlog.error("%s", msg.c_str());
        on_connection_error(msg);
        return false;
    }
}

bool VncConnection::processMessages()
{
    try {
        return processMsg();
    } catch (std::exception& e) {
        std::string msg = "Error processing messages for "
                          + serverHost_ + ":" + std::to_string(serverPort_)
                          + ": " + e.what();
        vlog.error("%s", msg.c_str());
        on_connection_error(msg);
        connected_ = false;
        return false;
    }
}

int VncConnection::getSocketFd() const
{
    if (sock_)
        return sock_->getFd();
    return -1;
}

bool VncConnection::isConnected() const
{
    return connected_;
}

void VncConnection::sendKeyPress(uint32_t keyCode, uint32_t keySym)
{
    CConnection::sendKeyPress(0, keyCode, keySym);
}

void VncConnection::sendKeyRelease(uint32_t keyCode)
{
    CConnection::sendKeyRelease(keyCode);
}

void VncConnection::sendPointer(int x, int y, uint8_t buttonMask)
{
    if (writer())
        writer()->writePointerEvent({x, y}, buttonMask);
}

void VncConnection::sendClipboard(const std::string& text)
{
    CConnection::sendClipboardData(text.c_str());
}

FramebufferInfo VncConnection::getFramebufferInfo() const
{
    FramebufferInfo info;
    if (pixelBuffer_) {
        info.width = pixelBuffer_->width();
        info.height = pixelBuffer_->height();
        info.stride = pixelBuffer_->width();
        int stride;
        const uint8_t* data = pixelBuffer_->getBuffer(
            pixelBuffer_->getRect(), &stride);
        info.data_ptr = reinterpret_cast<uint64_t>(data);
        info.stride = stride;
    } else {
        info.width = 0;
        info.height = 0;
        info.stride = 0;
        info.data_ptr = 0;
    }
    return info;
}

// --- CConnection callback implementations ---

void VncConnection::getUserPasswd(bool secure, std::string* user,
                                  std::string* password)
{
    auto result = on_get_credentials(secure);
    if (!result.ok) {
        throw std::runtime_error("Authentication cancelled by user");
    }
    if (user)
        *user = std::string(result.username);
    if (password)
        *password = std::string(result.password);
}

bool VncConnection::showMsgBox(rfb::MsgBoxFlags flags, const char* title,
                               const char* text)
{
    return on_show_message(static_cast<int32_t>(flags),
                           rust::Str(title), rust::Str(text));
}

void VncConnection::initDone()
{
    int w = server.width();
    int h = server.height();

    allocateFramebuffer(w, h);

    setPreferredEncoding(rfb::encodingTight);
    setCompressLevel(2);
    setQualityLevel(8);

    rfb::PixelFormat pf(32, 24, false, true, 255, 255, 255, 16, 8, 0);
    setPF(pf);

    on_init_done(w, h);
    vlog.info("Init done: %dx%d", w, h);
}

void VncConnection::resizeFramebuffer()
{
    int w = server.width();
    int h = server.height();
    allocateFramebuffer(w, h);
    on_init_done(w, h);
}

void VncConnection::allocateFramebuffer(int w, int h)
{
    rfb::PixelFormat pf(32, 24, false, true, 255, 255, 255, 16, 8, 0);

    auto* newBuffer = new rfb::ManagedPixelBuffer(pf, w, h);
    setFramebuffer(newBuffer);
    pixelBuffer_ = newBuffer;
}

void VncConnection::setName(const char* name)
{
    CConnection::setName(name);
    vlog.info("Server name: %s", name);
}

void VncConnection::bell()
{
    on_bell();
}

void VncConnection::framebufferUpdateStart()
{
    CConnection::framebufferUpdateStart();
}

void VncConnection::framebufferUpdateEnd()
{
    CConnection::framebufferUpdateEnd();

    if (pixelBuffer_) {
        DamageRect rect;
        rect.x = 0;
        rect.y = 0;
        rect.w = pixelBuffer_->width();
        rect.h = pixelBuffer_->height();
        on_frame_updated(rect);
    }
}

bool VncConnection::dataRect(const core::Rect& r, int encoding)
{
    if (encoding != rfb::encodingCopyRect)
        lastEncoding_ = encoding;
    return CConnection::dataRect(r, encoding);
}

void VncConnection::setCursor(int width, int height,
                              const core::Point& hotspot,
                              const uint8_t* data)
{
    CConnection::setCursor(width, height, hotspot, data);

    size_t dataLen = width * height * 4;
    rust::Slice<const uint8_t> slice(data, dataLen);
    on_cursor_changed(width, height, hotspot.x, hotspot.y, slice);
}

void VncConnection::setCursorPos(const core::Point& pos)
{
    // Cursor position tracking for the Rust UI
    (void)pos;
}

void VncConnection::setLEDState(unsigned int state)
{
    CConnection::setLEDState(state);
}

void VncConnection::handleClipboardRequest()
{
    // The server wants our clipboard data
}

void VncConnection::handleClipboardAnnounce(bool available)
{
    on_clipboard_announce(available);
}

void VncConnection::handleClipboardData(const char* data)
{
    on_clipboard_data(rust::Str(data));
}

void VncConnection::setExtendedDesktopSize(unsigned reason, unsigned result,
                                           int w, int h,
                                           const rfb::ScreenSet& layout)
{
    CConnection::setExtendedDesktopSize(reason, result, w, h, layout);
}

// --- C API for cxx bridge ---

std::unique_ptr<VncConnection> vnc_create()
{
    return std::make_unique<VncConnection>();
}

bool vnc_connect(VncConnection& conn, rust::Str host, int32_t port)
{
    return conn.connect(std::string(host), port);
}

bool vnc_process_messages(VncConnection& conn)
{
    return conn.processMessages();
}

int32_t vnc_get_socket_fd(const VncConnection& conn)
{
    return conn.getSocketFd();
}

bool vnc_is_connected(const VncConnection& conn)
{
    return conn.isConnected();
}

FramebufferInfo vnc_get_framebuffer_info(const VncConnection& conn)
{
    return conn.getFramebufferInfo();
}

void vnc_send_key_press(VncConnection& conn, uint32_t keyCode, uint32_t keySym)
{
    conn.sendKeyPress(keyCode, keySym);
}

void vnc_send_key_release(VncConnection& conn, uint32_t keyCode)
{
    conn.sendKeyRelease(keyCode);
}

void vnc_send_pointer(VncConnection& conn, int32_t x, int32_t y, uint8_t buttonMask)
{
    conn.sendPointer(x, y, buttonMask);
}

void vnc_send_clipboard(VncConnection& conn, rust::Str text)
{
    conn.sendClipboard(std::string(text));
}

void vnc_set_preferred_encoding(VncConnection& conn, int32_t encoding)
{
    conn.setPreferredEncoding(encoding);
}

void vnc_set_quality_level(VncConnection& conn, int32_t level)
{
    conn.setQualityLevel(level);
}

void vnc_set_compress_level(VncConnection& conn, int32_t level)
{
    conn.setCompressLevel(level);
}

void vnc_disconnect(VncConnection& conn)
{
    conn.close();
}

} // namespace vnc_bridge
