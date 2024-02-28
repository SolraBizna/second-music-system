#!/usr/bin/env bash

set -e

cd "$(dirname "$0")"

print_usage() {
    cat <<EOF

Usage: ./build.sh [--prefix </path/to/install>] [--target <target-triple>] [--nightly] [--build] [--install] [--debug]

 --prefix: Change where the library is installed. The library will be installed
           in "\${prefix}/lib" and the headers in "\${prefix}/include".
 --target: Change what target triple we're compiling for. Normally not required
           unless you also specify "--nightly".
--nightly: Use the nightly toolchain. This will result in a smaller library,
           but requires the nightly toolchain to be installed for the build
           target.
  --build: Build the library.
--install: Install the library.
  --debug: Build or install a debug version of the library, instead of a
           release version. The binary will be much larger and slower.

(If neither of --build or --install is specified, both will be performed.)

EOF
}

prefix=/usr/local
target=
nightly=
build=
install=

while [ $# -gt 0 ]; do
    case "$1" in
        --help)
            print_usage
            exit 0
            ;;
        --prefix)
            prefix="$2"
            if [ -z "$prefix" ]; then
                echo "You must specify an installation path after --prefix"
                print_usage
                exit 1
            fi
            shift 2
            ;;
        --target)
            target="$2"
            if [ -z "$target" ]; then
                echo "You must specify an host triple after --target"
                print_usage
                exit 1
            fi
            shift 2
            ;;
        --nightly)
            nightly=1
            shift
            ;;
        --build)
            build=1
            shift
            ;;
        --install)
            install=1
            shift
            ;;
        --debug)
            debug=1
            shift
            ;;
        *)
            echo "Unknown argument: $1"
            print_usage
            exit 1
    esac
done

if [ -z "$build" -a -z "$install" ]; then
    # If neither specified, do both.
    build=1
    install=1
fi

if [ -n "$build" ]; then
    if ! hash cargo 2>/dev/null || ! hash rustup 2>/dev/null; then
        echo "Please install a Rust environment, or, if it's already installed,"
        echo "ensure that \"cargo\" and \"rustup\" are in your PATH."
        echo ""
        echo "See <https://www.rust-lang.org/learn/get-started> for quick instructions."
        exit 1
    fi
fi

find_target() {
    if [ -n "$target" ]; then return; fi
    if hash rustup 2>/dev/null; then
        target="$(rustup show 2>/dev/null | awk '/^Default host: .*$/ { sub("Default host: ",""); print }')"
        echo "Detected target: $target"
        mkdir -p ../target
        echo "$target" > ../target/.detected-target
    elif [ -f ../target/.detected-target ]; then
        # we stashed this in case of "build && sudo install"
        target="$(cat ../target/.detected-target)"
    fi
    if [ -z "$target" ]; then
        echo "Unable to determine the default target for this Rust toolchain."
        echo "Please specify a target triple using \"--target\"."
        exit 1
    fi
}

buildcmd=(cargo)

if [ -n "$nightly" ]; then
    buildcmd+=(+nightly -Z build-std=std,panic_abort)
    find_target
fi

buildcmd+=(build)

if [ -n "$target" ]; then
    buildcmd+=(--target "$target")
fi

if [ -n "$debug" ]; then
    buildtype=debug
else
    buildtype=release
    buildcmd+=(--release)
fi

if [ -n "$build" ]; then
    RUSTFLAGS="-C panic=abort" ${buildcmd[@]}
fi

if [ -n "$install" ]; then
    libfile="../target/$target/$buildtype/libc_second_music_system.a"
    if [ ! -f "$libfile" ]; then
        echo "$libfile: not found"
        echo "It looks like the library hasn't been built. Please build it"
        echo "before installing."
        exit 1
    fi
    mkdir -p "$prefix/lib"
    cp -v "$libfile" "$prefix/lib/libc_second_music_system.a"
    mkdir -p "$prefix/include"
    cp -v include/*.h "$prefix/include/"
fi
