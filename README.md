# lulu

Concept of package manager built on top of apt for handling git repositories.
It's highly inspired from Arch Linux and was designed to make easy the installation of programs which require an infinite
number of dependencies.

A use case: the latest version of [awesomewm](https://github.com/awesomeWM/awesome) (at the time of February 2023) was
released in January 2019.
Since that release there were 1588 commits which adds a lot of functionalities.

_Note: There are several functionalities which are not implement like handling dependencies with other git repositories, 
handling ssh, ..._

You can find builds of lulu in the [actions tab](https://github.com/alyrow/lulu/actions) for ubuntu 20.04 and ubuntu 22.04.

If you want to build and install yourself lulu, you need to install the following dependencies:

- libapt-pkg-dev
- gcc
- pkg-config
- libssl-dev
- g++
- fakeroot
- libapt-pkg6.0

```shell
# apt install libapt-pkg-dev gcc pkg-config libssl-dev g++ fakeroot libapt-pkg6.0
```

[Setup a rust environment](https://rustup.rs/)

Then run:

```shell
git clone https://github.com/alyrow/lulu.git
cd lulu

cargo run -- setup # Init lulu db
cargo run -- install # Build and install lulu
```

## Usage

```
Usage: lulu [OPTIONS] [COMMAND]

Commands:
  install  Install packages
  setup    Setup lulu db
  update   Update each repository and eventually inform about possible upgrades
  upgrade  Upgrade installed packages
  remove   Remove an installed package
  list     List packages
  help     Print this message or the help of the given subcommand(s)

Options:
      --no-color  Disable color output
  -d, --debug     Enable debug mode
  -h, --help      Print help
  -V, --version   Print version

```

### Setup command

```shell
$ lulu setup
```

You should run this command only once (normally done when installing .deb).
It initializes the database in `/var/lib/lulu/db`.

### Install command

Install a package from the `LULU.toml` file in the current directory:

```shell
$ lulu install
```

Install a package from a git repository with a `LULU.toml` file:

```shell
$ lulu install https://github.com/alyrow/lulu.git
```

Install a package from a lulu repository:

```shell
$ lulu install package-name # lulu-git or awesome-git
```

### Update repositories

```shell
$ lulu update
```

It works like apt.

### Upgrade installed packages

```shell
$ lulu upgrade
```

_Note: it only upgrades a package if present in a lulu repository_

### Remove a package

```shell
$ lulu remove package-name
```

If you want to purge the package:

```shell
$ lulu remove -p package-name
```

### List package

```shell
$ lulu list
```

If you want to list installed packages:

```shell
$ lulu list -i
```

## Configuration

You can find the lulu config file at `/etc/lulu.conf`.
It's a toml file which looks like that:

```toml
ignore = []

[[repositories]]
name = "main"
source = "https://github.com/alyrow/lulu-packages.git"
```

`ignore` field is for ignoring package upgrade.

And `repositories` section is for adding lulu repositories which take a **unique** name and a source (git url).

## LULU.toml

Maybe you are interested to package other git repositories, so you need to create a `LULU.toml` file.

The structure of a `LULU.toml` file looks like that:

```toml
[package]
name = "lulu-git" # Name of the package, we will prefer to add `-git` suffix to differenciate with the package provided by apt
maintainers = ["alyrow"] # Mainteners name
description = "Concept of package manager built on top of apt for handling git repositories" # A description of the programm you package
url = "https://github.com/alyrow/lulu" # [Optionnal] Url to the programm website
source = "https://github.com/alyrow/lulu.git" # Git url of the source repository
arch = ["any"] # Architecture supported (Not used actually)
license = [] # License of the programm
provides = ["lulu"] # What programm(s) provide the package
preinst = "" # [Optionnal] A script run before installation of the package
# [Optionnal] A script run after installation of the package
postinst = ''' 
#!/bin/sh
set -e

lulu setup
'''
prerm = "" # [Optionnal] A script run before removal of the package
postrm = "" # [Optionnal] A script run after removal of the package

# Dependencies section (note: git dependencies are not implemented!)

# Dependencies needed by your programm
[dependencies.runtime]
fakeroot = { is = "APT" }
"libapt-pkg6.0" = { is = "APT" }

# Dependencies needed to build your programm
[dependencies.build]
libapt-pkg-dev = { is = "APT" }
gcc = { is = "APT" }
pkg-config = { is = "APT" }
libssl-dev = { is = "APT" }
"g++" = { is = "APT" }

# Optional dependencies needed by your programm
[dependencies.optional]

[script]
# Prepare script usually used to move files before building
prepare = '''
'''
# Build script run for building your programm
build = '''
export PATH=$PATH:$HOME/.cargo/bin/
cargo update
cargo build --release
'''
# Check script run for running test for example
check = '''

'''
# Package script which is run with fakeroot for installing your programm in $pkgdir folder in order to package it
package = '''
mkdir -p $pkgdir/usr/bin
mkdir -p $pkgdir/etc
install -Dm755 target/release/lulu $pkgdir/usr/bin/lulu
install -Dm644 lulu.conf $pkgdir/etc/lulu.conf
'''
```

Note: The following shell variables are available in scripts:

- `$srcdir`: Source files
- `$basedir`: Where your `LULU.toml` file is
- `$pkgdir`: Dir where you will put your files to be packaged

Some useful references:

- https://wiki.archlinux.org/title/Creating_packages#PKGBUILD_functions
