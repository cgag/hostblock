Simple terminal interface for blocking websites via the /etc/hosts file.

![Hostblock demo](http://curtis.io/img/hostblock-cropped.gif "Hostblock Demo")

When you unblock or a remove a domain you'll be asked to enter a mildly
annoying random passphrase to give you a chance to reconsider if you really
want to read go read reddit.

Must be run as sudo as it needs to write to /etc/hosts.

Controls
  - i 		- add a new domain
  - j/k 	- down, up
  - J/K 	- goto bottom, goto top
  - d 		- delete selected
  - space - toggle whether or not selected domain is blocked
  - q     - Quit current mode, quits app if in normal mode.
  - h     - View help.

### Installation:

#### Linux x86_64 binary
  If you're on linux on an x86_64 processor you can download a binary release
  here:  https://github.com/cgag/hostblock/releases.

  The binary is fully statically linked against musl-libc, so it should work
  on any distro.

#### Building from source
  You'll need to have rustc and cargo installed, you can download them here:
    [https://www.rust-lang.org/install.html](https://www.rust-lang.org/install.html).
    Note that cargo is bundled with rust.

	- clone repo
	- run `cargo build --release`
	- sudo ./target/release/hostblock

  You should probably move the binary to somewhere on your path.
