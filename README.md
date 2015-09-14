Simple terminal interface for blocking websites via the /etc/hosts file.

![Hostblock demo](http://curtis.io/img/hostblock-cropped.gif "Hostblock Demo")

Must be run as sudo as it needs to write to the hosts file, plus a backup
file at /etc/hosts.hb.back

Controls
  - i 		- add a new domain
  - j/k 	- down, up
  - J/K 	- goto bottom, goto top
  - d 		- delete selected
  - space - toggle whether or not selected domain is blocked
  - q     - Quit current mode, quits app if in normal mode.

### Installation:

#### Building from source
  You'll need to have rustc and cargo installed, you can download them here:
    [https://www.rust-lang.org/install.html](https://www.rust-lang.org/install.html).
    Note that cargo is bundled with rust.
	- clone repo
	- run `cargo build --release`
	- sudo ./target/release/hostblock
  You should probably must move the binary to somewhere on your path.
