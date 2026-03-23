// RuVNC Viewer - Modern Rust/egui VNC viewer
// Copyright (C) 2026 BackBenchDevs
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use std::path::{Path, PathBuf};

struct BuildTarget {
    is_windows: bool,
    is_unix: bool,
    is_macos: bool,
}

struct LibFlags {
    include_paths: Vec<PathBuf>,
    link_paths: Vec<PathBuf>,
    has_gnutls: bool,
    has_nettle: bool,
    target: BuildTarget,
}

fn generate_config_h(out_dir: &Path, cmake_build_dir: &Path) {
    let config_dst = out_dir.join("config.h");
    let cmake_config = cmake_build_dir.join("config.h");
    if cmake_config.exists() {
        std::fs::copy(&cmake_config, &config_dst)
            .expect("Failed to copy config.h from CMake build");
    } else {
        std::fs::write(
            &config_dst,
            r#"#define PACKAGE_NAME "tigervnc"
#define PACKAGE_VERSION "1.16.80"

#if defined(HAVE_GNUTLS) && defined(WIN32) && !defined(__MINGW32__)
    #if defined(_WIN64)
        typedef __int64 ssize_t;
    #else
        typedef long ssize_t;
    #endif
#endif

#if defined(__APPLE__) && defined(__clang__)
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
#endif
"#,
        )
        .expect("Failed to write config.h");
    }
}

struct PkgInfo {
    include_paths: Vec<PathBuf>,
    link_paths: Vec<PathBuf>,
}

fn probe_lib(name: &str) -> PkgInfo {
    let mut cfg = pkg_config::Config::new();
    cfg.cargo_metadata(false);
    match cfg.probe(name) {
        Ok(lib) => PkgInfo {
            include_paths: lib.include_paths,
            link_paths: lib.link_paths,
        },
        Err(_) => PkgInfo {
            include_paths: Vec::new(),
            link_paths: Vec::new(),
        },
    }
}

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_family = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    let target = BuildTarget {
        is_windows: target_os == "windows",
        is_unix: target_family == "unix",
        is_macos: target_os == "macos",
    };

    let tigervnc_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let common_dir = tigervnc_root.join("common");
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bridge");

    let has_gnutls = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("gnutls")
        .is_ok();
    let has_nettle = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("nettle")
        .is_ok();

    let mut include_paths = Vec::new();
    let mut link_paths = Vec::new();

    for name in &["zlib", "pixman-1", "libjpeg"] {
        let info = probe_lib(name);
        include_paths.extend(info.include_paths);
        link_paths.extend(info.link_paths);
    }
    if has_gnutls {
        let info = probe_lib("gnutls");
        include_paths.extend(info.include_paths);
        link_paths.extend(info.link_paths);
    }
    if has_nettle {
        for name in &["nettle", "gmp"] {
            let info = probe_lib(name);
            include_paths.extend(info.include_paths);
            link_paths.extend(info.link_paths);
        }
    }

    let flags = LibFlags {
        include_paths,
        link_paths,
        has_gnutls,
        has_nettle,
        target,
    };

    build_bridge(&common_dir, &bridge_dir, &flags);
    build_vnc_core(&common_dir, &flags);
}

fn apply_common_flags(build: &mut cc::Build, common_dir: &Path, out_dir: &Path, flags: &LibFlags) {
    build
        .cpp(true)
        .std("c++14")
        .include(common_dir)
        .include(out_dir)
        .warnings(false)
        .define("PACKAGE_NAME", "\"tigervnc\"")
        .define("PACKAGE_VERSION", "\"1.16.80\"")
        .define("BUILD_TIMESTAMP", "\"ruvnc-viewer\"")
        .define("HAVE_CONFIG_H", None);

    if flags.target.is_windows {
        build.define("WIN32", None);
    }
    if flags.has_gnutls {
        build.define("HAVE_GNUTLS", None);
    }
    if flags.has_nettle {
        build.define("HAVE_NETTLE", None);
    }
    for inc in &flags.include_paths {
        build.include(inc);
    }
}

fn build_vnc_core(common_dir: &Path, flags: &LibFlags) {
    let core_dir = common_dir.join("core");
    let rdr_dir = common_dir.join("rdr");
    let network_dir = common_dir.join("network");
    let rfb_dir = common_dir.join("rfb");
    let tigervnc_root = common_dir.parent().unwrap();
    let cmake_build_dir = tigervnc_root.join("build");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    generate_config_h(&out_dir, &cmake_build_dir);

    let mut build = cc::Build::new();
    apply_common_flags(&mut build, common_dir, &out_dir, flags);

    let core_sources = [
        "Configuration.cxx",
        "Exception.cxx",
        "Logger.cxx",
        "Logger_file.cxx",
        "Logger_stdio.cxx",
        "LogWriter.cxx",
        "Region.cxx",
        "Timer.cxx",
        "string.cxx",
        "time.cxx",
        "xdgdirs.cxx",
    ];
    for src in &core_sources {
        build.file(core_dir.join(src));
    }
    if flags.target.is_unix {
        build.file(core_dir.join("Logger_syslog.cxx"));
    }

    let rdr_sources = [
        "AESInStream.cxx",
        "AESOutStream.cxx",
        "BufferedInStream.cxx",
        "BufferedOutStream.cxx",
        "FdInStream.cxx",
        "FdOutStream.cxx",
        "FileInStream.cxx",
        "HexInStream.cxx",
        "HexOutStream.cxx",
        "RandomStream.cxx",
        "TLSException.cxx",
        "TLSInStream.cxx",
        "TLSOutStream.cxx",
        "TLSSocket.cxx",
        "ZlibInStream.cxx",
        "ZlibOutStream.cxx",
    ];
    for src in &rdr_sources {
        build.file(rdr_dir.join(src));
    }

    build.file(network_dir.join("Socket.cxx"));
    build.file(network_dir.join("TcpSocket.cxx"));
    if flags.target.is_unix {
        build.file(network_dir.join("UnixSocket.cxx"));
    }

    let rfb_sources = [
        "AccessRights.cxx",
        "Blacklist.cxx",
        "Congestion.cxx",
        "CConnection.cxx",
        "CMsgReader.cxx",
        "CMsgWriter.cxx",
        "CSecurityPlain.cxx",
        "CSecurityStack.cxx",
        "CSecurityVeNCrypt.cxx",
        "CSecurityVncAuth.cxx",
        "ClientParams.cxx",
        "ComparingUpdateTracker.cxx",
        "CopyRectDecoder.cxx",
        "Cursor.cxx",
        "DecodeManager.cxx",
        "Decoder.cxx",
        "d3des.c",
        "EncodeManager.cxx",
        "Encoder.cxx",
        "HextileDecoder.cxx",
        "HextileEncoder.cxx",
        "JpegCompressor.cxx",
        "JpegDecompressor.cxx",
        "JPEGDecoder.cxx",
        "JPEGEncoder.cxx",
        "KeyRemapper.cxx",
        "KeysymStr.c",
        "PixelBuffer.cxx",
        "PixelFormat.cxx",
        "RREEncoder.cxx",
        "RREDecoder.cxx",
        "RawDecoder.cxx",
        "RawEncoder.cxx",
        "SConnection.cxx",
        "SMsgReader.cxx",
        "SMsgWriter.cxx",
        "ServerCore.cxx",
        "ServerParams.cxx",
        "Security.cxx",
        "SecurityServer.cxx",
        "SecurityClient.cxx",
        "SSecurityPlain.cxx",
        "SSecurityStack.cxx",
        "SSecurityVncAuth.cxx",
        "SSecurityVeNCrypt.cxx",
        "TightDecoder.cxx",
        "TightEncoder.cxx",
        "TightJPEGEncoder.cxx",
        "UpdateTracker.cxx",
        "VNCSConnectionST.cxx",
        "VNCServerST.cxx",
        "ZRLEEncoder.cxx",
        "ZRLEDecoder.cxx",
        "encodings.cxx",
        "obfuscate.cxx",
    ];

    let mut c_build = cc::Build::new();
    c_build.include(common_dir).warnings(false);
    let mut has_c_files = false;

    for src in &rfb_sources {
        if src.ends_with(".c") {
            c_build.file(rfb_dir.join(src));
            has_c_files = true;
        } else {
            build.file(rfb_dir.join(src));
        }
    }

    if flags.has_gnutls {
        build.file(rfb_dir.join("CSecurityTLS.cxx"));
        build.file(rfb_dir.join("SSecurityTLS.cxx"));
    }
    if flags.has_nettle {
        build.file(rfb_dir.join("CSecurityDH.cxx"));
        build.file(rfb_dir.join("CSecurityMSLogonII.cxx"));
        build.file(rfb_dir.join("CSecurityRSAAES.cxx"));
        build.file(rfb_dir.join("SSecurityRSAAES.cxx"));
    }
    if flags.target.is_unix && !flags.target.is_macos {
        build.file(rfb_dir.join("UnixPasswordValidator.cxx"));
    }

    for path in &flags.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }

    // On Windows (MinGW), the GNU linker is single-pass and sensitive to
    // library ordering.  Wrap all our static libs and their dependencies in a
    // linker group so cross-references resolve regardless of emission order.
    if flags.target.is_windows {
        println!("cargo:rustc-link-arg=-Wl,--start-group");
    }

    build.compile("vnccore");
    println!("cargo:rustc-link-lib=static=vnccore");

    if has_c_files {
        c_build.compile("vnccore_c");
        println!("cargo:rustc-link-lib=static=vnccore_c");
    }

    println!("cargo:rustc-link-lib=pixman-1");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=jpeg");
    if flags.target.is_unix {
        println!("cargo:rustc-link-lib=pthread");
    }
    if flags.has_gnutls {
        println!("cargo:rustc-link-lib=gnutls");
    }
    if flags.has_nettle {
        println!("cargo:rustc-link-lib=nettle");
        println!("cargo:rustc-link-lib=hogweed");
        println!("cargo:rustc-link-lib=gmp");
    }
    if flags.target.is_unix && !flags.target.is_macos {
        println!("cargo:rustc-link-lib=pam");
    }
    if flags.target.is_windows {
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=secur32");
    }
    if flags.target.is_macos {
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }

    if flags.target.is_windows {
        println!("cargo:rustc-link-arg=-Wl,--end-group");
    }
}

fn build_bridge(common_dir: &Path, bridge_dir: &Path, flags: &LibFlags) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let sources = vec![bridge_dir.join("src").join("headless_conn.cc")];

    let mut builder = cxx_build::bridge("src/bridge.rs");
    apply_common_flags(&mut builder, common_dir, &out_dir, flags);
    builder
        .files(sources)
        .include(bridge_dir.join("include"))
        .compile("vnc_bridge");

    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=bridge/src/headless_conn.cc");
    println!("cargo:rerun-if-changed=bridge/include/headless_conn.h");
}
