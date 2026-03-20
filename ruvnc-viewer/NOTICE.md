# RuVNC Viewer - Legal Notice

## Derivative Work

This project is a derivative work based on [TigerVNC](https://github.com/TigerVNC/tigervnc),
a high-performance, platform-neutral implementation of VNC (Virtual Network Computing).

RuVNC Viewer provides a new user interface built in Rust using the egui framework,
which interfaces with the original TigerVNC C++ core protocol engine via a cxx bridge.

## Modifications

- **Modified by:** BackBenchDevs
- **Date of modification:** March 2026
- **Summary of changes:**
  - Added a custom Rust/egui UI layer replacing the original FLTK-based viewer
  - Created C++/Rust bindings (cxx bridge) to the TigerVNC common/ libraries
  - Added an address book with persistent server management
  - Added secure credential storage via OS keyring
  - Added team server synchronization

## License

This program is free software; you can redistribute it and/or modify it under the
terms of the GNU General Public License as published by the Free Software Foundation;
either version 2 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this
program. If not, see <https://www.gnu.org/licenses/>.

## Source Code

The complete source code for this program is available at:
<https://github.com/BackBenchDevs/tigervnc>

## Original Copyright

The original TigerVNC code is Copyright (C) 1999-2025 by the TigerVNC Team and
its contributors (RealVNC Ltd, TightVNC Team, Cendio AB, and others).
See individual source files for specific copyright holders.
