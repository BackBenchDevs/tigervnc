// HeadlessCConn: FLTK-free CConnection subclass for the Rust bridge.
// This replaces vncviewer/CConn.h without any UI toolkit dependency.

#ifndef __HEADLESS_CONN_H__
#define __HEADLESS_CONN_H__

#include <memory>
#include <string>
#include <cstdint>

#include <rfb/CConnection.h>
#include <rfb/PixelBuffer.h>
#include <network/TcpSocket.h>

#include "rust/cxx.h"

namespace vnc_bridge {

struct DamageRect;
struct CredentialResult;
struct FramebufferInfo;

class VncConnection : public rfb::CConnection {
public:
    VncConnection();
    ~VncConnection() override;

    bool connect(const std::string& host, int port);
    void disconnect();
    bool processMessages();
    int getSocketFd() const;
    bool isConnected() const;

    void sendKeyPress(uint32_t keyCode, uint32_t keySym);
    void sendKeyRelease(uint32_t keyCode);
    void sendPointer(int x, int y, uint8_t buttonMask);
    void sendClipboard(const std::string& text);

    FramebufferInfo getFramebufferInfo() const;

protected:
    // CConnection pure virtual overrides
    void getUserPasswd(bool secure, std::string* user,
                       std::string* password) override;
    bool showMsgBox(rfb::MsgBoxFlags flags, const char* title,
                    const char* text) override;
    void initDone() override;
    void resizeFramebuffer() override;

    // CMsgHandler overrides
    void setName(const char* name) override;
    void bell() override;
    void framebufferUpdateStart() override;
    void framebufferUpdateEnd() override;
    bool dataRect(const core::Rect& r, int encoding) override;
    void setCursor(int width, int height, const core::Point& hotspot,
                   const uint8_t* data) override;
    void setCursorPos(const core::Point& pos) override;
    void setLEDState(unsigned int state) override;
    void handleClipboardRequest() override;
    void handleClipboardAnnounce(bool available) override;
    void handleClipboardData(const char* data) override;
    void setExtendedDesktopSize(unsigned reason, unsigned result,
                                int w, int h,
                                const rfb::ScreenSet& layout) override;

private:
    void allocateFramebuffer(int w, int h);

    network::Socket* sock_;
    bool connected_;
    std::string serverHost_;
    int serverPort_;

    rfb::ManagedPixelBuffer* pixelBuffer_;
    int lastEncoding_;
};

// C API for cxx bridge
std::unique_ptr<VncConnection> vnc_create();
bool vnc_connect(VncConnection& conn, rust::Str host, int32_t port);
bool vnc_process_messages(VncConnection& conn);
int32_t vnc_get_socket_fd(const VncConnection& conn);
bool vnc_is_connected(const VncConnection& conn);
FramebufferInfo vnc_get_framebuffer_info(const VncConnection& conn);
void vnc_send_key_press(VncConnection& conn, uint32_t keyCode, uint32_t keySym);
void vnc_send_key_release(VncConnection& conn, uint32_t keyCode);
void vnc_send_pointer(VncConnection& conn, int32_t x, int32_t y, uint8_t buttonMask);
void vnc_send_clipboard(VncConnection& conn, rust::Str text);
void vnc_set_preferred_encoding(VncConnection& conn, int32_t encoding);
void vnc_set_quality_level(VncConnection& conn, int32_t level);
void vnc_set_compress_level(VncConnection& conn, int32_t level);
void vnc_disconnect(VncConnection& conn);

} // namespace vnc_bridge

#endif // __HEADLESS_CONN_H__
