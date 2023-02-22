# lulu

Concept of package manager built on top of apt for handling git repositories.
It's highly inspired from Arch Linux and was designed to make easy the installation of programs which require an infinite
number of dependencies.

A use case: the latest version of [awesomewm](https://github.com/awesomeWM/awesome) (at the time of February 2023) was
released in January 2019.
Since that release there are 1588 commits which adds a lot of functionalities.

_Note: There are several functionalities which are not implement like handling dependencies with other git repositories, 
handling ssh, ..._

You can find builds of lulu in the [actions tab](https://github.com/alyrow/lulu/actions) for ubuntu 20.04 and ubuntu 22.04.

If you want to build and install yourself lulu, you need to install the following dependencies:

- libapt-pkg-dev

[Setup a rust environment](https://rustup.rs/)

Then run:

```shell
git clone https://github.com/alyrow/lulu.git
cd lulu

cargo run -- setup # Init lulu db
cargo run --verbose -- install # Build and install lulu
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
