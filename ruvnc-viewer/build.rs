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

fn main() {
    let tigervnc_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let common_dir = tigervnc_root.join("common");
    let bridge_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bridge");

    build_vnc_core(&common_dir);
    build_bridge(&common_dir, &bridge_dir);
}

fn build_vnc_core(common_dir: &Path) {
    let core_dir = common_dir.join("core");
    let rdr_dir = common_dir.join("rdr");
    let network_dir = common_dir.join("network");
    let rfb_dir = common_dir.join("rfb");
    let tigervnc_root = common_dir.parent().unwrap();
    let cmake_build_dir = tigervnc_root.join("build");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    generate_config_h(&out_dir, &cmake_build_dir);

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
        .include(common_dir)
        .include(&out_dir)
        .warnings(false)
        .define("PACKAGE_NAME", "\"tigervnc\"")
        .define("PACKAGE_VERSION", "\"1.16.80\"")
        .define("BUILD_TIMESTAMP", "\"ruvnc-viewer\"")
        .define("HAVE_CONFIG_H", None);

    if cfg!(target_os = "windows") {
        build.define("WIN32", None);
    }

    if let Ok(lib) = pkg_config::probe_library("zlib") {
        for inc in &lib.include_paths {
            build.include(inc);
        }
    }
    if let Ok(lib) = pkg_config::probe_library("pixman-1") {
        for inc in &lib.include_paths {
            build.include(inc);
        }
    }
    if let Ok(lib) = pkg_config::probe_library("libjpeg") {
        for inc in &lib.include_paths {
            build.include(inc);
        }
    }

    let has_gnutls = pkg_config::probe_library("gnutls").is_ok();
    if has_gnutls {
        build.define("HAVE_GNUTLS", None);
        if let Ok(lib) = pkg_config::probe_library("gnutls") {
            for inc in &lib.include_paths {
                build.include(inc);
            }
        }
    }

    let has_nettle = pkg_config::probe_library("nettle").is_ok();
    if has_nettle {
        build.define("HAVE_NETTLE", None);
        if let Ok(lib) = pkg_config::probe_library("nettle") {
            for inc in &lib.include_paths {
                build.include(inc);
            }
        }
        if let Ok(lib) = pkg_config::probe_library("gmp") {
            for inc in &lib.include_paths {
                build.include(inc);
            }
        }
    }

    // core sources
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
    if cfg!(unix) {
        build.file(core_dir.join("Logger_syslog.cxx"));
    }

    // rdr sources
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

    // network sources
    build.file(network_dir.join("Socket.cxx"));
    build.file(network_dir.join("TcpSocket.cxx"));
    if cfg!(unix) {
        build.file(network_dir.join("UnixSocket.cxx"));
    }

    // rfb sources
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
    for src in &rfb_sources {
        if src.ends_with(".c") {
            let mut c_build = cc::Build::new();
            c_build
                .include(common_dir)
                .warnings(false)
                .file(rfb_dir.join(src));
            c_build.compile(&format!("rfb_{}", src.replace('.', "_")));
        } else {
            build.file(rfb_dir.join(src));
        }
    }

    if has_gnutls {
        build.file(rfb_dir.join("CSecurityTLS.cxx"));
        build.file(rfb_dir.join("SSecurityTLS.cxx"));
    }
    if has_nettle {
        build.file(rfb_dir.join("CSecurityDH.cxx"));
        build.file(rfb_dir.join("CSecurityMSLogonII.cxx"));
        build.file(rfb_dir.join("CSecurityRSAAES.cxx"));
        build.file(rfb_dir.join("SSecurityRSAAES.cxx"));
    }
    if cfg!(unix) && !cfg!(target_os = "macos") {
        build.file(rfb_dir.join("UnixPasswordValidator.cxx"));
    }

    build.compile("vnccore");

    println!("cargo:rustc-link-lib=static=vnccore");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=jpeg");
    if cfg!(unix) {
        println!("cargo:rustc-link-lib=pthread");
    }
    if has_gnutls {
        println!("cargo:rustc-link-lib=gnutls");
    }
    if has_nettle {
        println!("cargo:rustc-link-lib=nettle");
        println!("cargo:rustc-link-lib=hogweed");
        println!("cargo:rustc-link-lib=gmp");
    }
    if cfg!(unix) && !cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=pam");
    }
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=secur32");
    }
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}

fn build_bridge(common_dir: &Path, bridge_dir: &Path) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let sources = vec![bridge_dir.join("src").join("headless_conn.cc")];

    cxx_build::bridge("src/bridge.rs")
        .files(sources)
        .include(common_dir)
        .include(&out_dir)
        .include(bridge_dir.join("include"))
        .std("c++14")
        .warnings(false)
        .define("HAVE_CONFIG_H", None)
        .define("PACKAGE_NAME", "\"tigervnc\"")
        .define("PACKAGE_VERSION", "\"1.16.80\"")
        .define("BUILD_TIMESTAMP", "\"ruvnc-viewer\"")
        .compile("vnc_bridge");

    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=bridge/src/headless_conn.cc");
    println!("cargo:rerun-if-changed=bridge/include/headless_conn.h");
}
