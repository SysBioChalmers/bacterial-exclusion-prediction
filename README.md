# Bacterial exclusion prediction

A program for estimating the anti-bacterial property of surfaces based on SEM
images.

This repository is administered by Shadi Rahimi (@Shadirahimi), Division of
Systems and Synthetic Biology, Department of Biology and Biological Engineering,
Chalmers University of Technology.

## Usage

The software can be build and run without arguments using Cargo if all
dependencies are installed (see "Build" section below for information about
installing dependencies).

```sh
cargo run --release
```

This command will start a interactive web server for live configuration locally
on http://127.0.0.1:8080. The interface greets the user with an analysis of the
image specified in the first text field.

To learn more about the other options to the program we can run the following
command, displaying a help menu describing the modes, flags and options.

```sh
cargo run --release -- --help
```

## Build

### Linux

The easiest way to compile the program is through [Nix](https://nixos.org/) with
the provided `shell.nix` file. After installing Nix you only have to execute the
following two commands from the cloned repository to get the program built and
run it.

```sh
nix-shell
cargo run --release
```

On other distributions you have to use your package manager for installation of
Tesseract (a text recognition library). Depending on the package manager used by
your distribution one of the following commands should install Tesseract when
executed with root privileges.

```sh
apt install tesseract-ocr     # Debian / Ubuntu based distributions
dnf install tesseract         # Fedora / RHEL based distributions
pacman -S tesseract           # Arch Linux based distributions
zypper install tesseract-ocr  # OpenSUSE based distributions
xbps-install -S tesseract-ocr # Void Linux based distributions
```

When Tesseract has been installed you have to install [Cargo](https://www.rust-lang.org/).
Read the install instruction for your particular situation but in short you can
execute the following command for installation of Rustup

```sh
curl https://sh.rustup.rs -sSf | sh
```

There after you should be ready to build the program like other Rust projects
using Cargo.

```sh
cargo run --release
```

### MacOS

We start by launching a terminal window through Launchpad by searching for
“terminal” in the search bar. Then we need to install Tesseract, which is used
for text recognition. To make this easier we first install Homebrew, a MacOS
package manager, by executing their installation script.

```sh
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Beware the command above downloads and executes code from the internet. After
Homebrew is installed by following the prompts, we can install Tesseract using
the following command.

```sh
brew install tesseract
```

Once Tesseract is installed we have to install Cargo which is the Rust build
system. It can be installed through the following command in the terminal.

```sh
curl https://sh.rustup.rs -sSf | sh
```

After that use Cargo to build and run the program when inside of the cloned git
repository.

```sh
cargo run --release
```

### Windows

We first install Tesseract, the text recognition software used by the program.
A Windows installer can be downloaded from https://github.com/UB-Mannheim/tesseract/wiki.
Run the installer with default settings and wait for it to finish.

Once Tesseract is installed, we need to open Powershell by searching for
"Powershell" in the search bar of Windows. Every time we open a new Powershell
window where we want to run the program we have to add Tesseract to our PATH
using the following command, where the you write the path to the Tesseract
installation.

```ps
$env:PATH += ";C:\Algorithm Files\Tesseract-OCR\;"
```

Next, Cargo has to be installed. The easiest way is through the Windows
installer of Rustup downloaded from [here](https://win.rustup.rs/) as as stated
in the [install instructions](https://doc.rust-lang.org/cargo/getting-started/installation.html).
After completing the installer Rust and therefore also Cargo should be
installed on your system.

Finally, enter the directory containing the cloned repository to be able to
build and run the software.

```
cargo run --release
```
