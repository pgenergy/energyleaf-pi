#!/usr/bin/env sh

INS_PATH=/usr/local

PB_REL="https://github.com/protocolbuffers/protobuf/releases"

PROTOC_PATH="$INS_PATH"/bin/protoc

# Get system architecture
ARCH=$(uname -m)

# if protoc path not exists
if [ ! -f "$PROTOC_PATH" ]; then
    echo "PROTOC not found, installing..."

    mkdir -p "$INS_PATH"

    if [ "$ARCH" = "x86_64" ]; then
        # Download x86_64 binary
        wget -O "$INS_PATH"/protoc.zip $PB_REL/download/v27.0/protoc-27.0-linux-x86_64.zip
    elif [ "$ARCH" = "aarch64" ]; then
        # Download aarch64 binary
        wget -O "$INS_PATH"/protoc.zip $PB_REL/download/v27.0/protoc-27.0-linux-aarch_64.zip
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi

    # Unzip the binary
    unzip "$INS_PATH"/protoc.zip -d "$INS_PATH"

    # Remove stale files
    rm -f "$INS_PATH"/protoc.zip
    rm -f "$INS_PATH"/readme.txt
fi

