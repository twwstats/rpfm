# Rusted PackFile Manager
***Rusted PackFile Manager*** (RPFM) is a... reimplementation in Rust and GTK3 of ***PackFile Manager*** (PFM), one of the best modding tools for Total War Games.

# Status of the Project
The lastest released version can (only for TW:Warhammer 2 for now):
- [x] ***Manipulate PackFiles*** (create, open and save).
- [x] ***Manipulate PackedFiles*** (add, extract, rename, delete).
- [x] ***Edit DB PackedFiles***.
- [x] ***Edit Localisation PackedFiles***.

In addition to that, it has some special features:
- [x] ***Patch SiegeAI***: For mappers. It patches the opened PackFile so the Siege AI stuff of your map works properly, delete the useless xml files in your map's folder and save your PackFile.

To see what's being worked on, check the ***TODO.md*** file.

# Requeriments (to use)
To use the ***Linux*** you just need to install **GTK 3.22 or later** and... that's all.

To use the ***Windows*** version, there are no requirements. Just extract it somewhere and execute it.


# Requeriments (to build)
To build this project, you need:
* ***GTK 3.22*** or higher.
* ***Rust toolchain***.

In **Linux**, you just need to install the lastest ***GTK3 package*** of your distro (at least GTK 3.22) and the ***Rust Toolchain*** (recommended using [***Rustup***][Rustup download]).

In **Windows**, first you need to install the ***Rust Toolchain*** (recommended using [***Rustup***][Rustup download]), then go [here][Gtk-rs requeriments] to learn the painful odyssey of installing GTK3 for Windows.

To build, move to the repo directory and execute:
```bash
# For Windows
cargo build --target=x86_64-pc-windows-gnu --release

# For Linux
cargo build --target=x86_64-unknown-linux-gnu --release
```

# FAQ
- **Why not helping with PFM instead of reimplement it?**: because I wanted to learn a new language, and I already now a bit of C#. Also, where is the fun of that?
- **Why the code quality is not the very best?**: because I'm using this project to learn Rust, and I'm constantly rewriting code as I find new and better ways to write it.

# Known bugs
- Float fields in DB PackedFiles have trailing zeros.
- Sometimes when selecting a DB PackedFile after having another one selected, the scrollbar disappear.
- Decoding of big Loc files (like the vanilla localisation file) is SLOOOOWWWWWW. Need to improve his decoding in the future.

# Credits
- ***Frodo45127***: I'm the guy who has made the program.
- ***Maruka*** (From Steam): He made the wazard hat's icon.

#### Special Thanks to:
- ***The guys that made PFM***: Most of the decoding stuff would have been very hard to work out without the PFM source code. So many thanks for make the PFM open source!

[Rustup download]: https://www.rustup.rs/ "Here you can download it :)"
[Gtk-rs requeriments]: http://gtk-rs.org/docs-src/requirements.html "Installation Tutorial for GTK3 in Windows"
