#!/usr/bin/env sh

INS_PATH=/usr/local

PB_REL="https://github.com/protocolbuffers/protobuf/releases"

PROTOC_PATH="$INS_PATH"/bin/protoc

# if protoc path not exists
if [ ! -f "$PROTOC_PATH" ]; then
    echo "PROTOC not found, installing..."
    mkdir -p "$INS_PATH"
    # Download the binary
    wget -O "$INS_PATH"/protoc.zip $PB_REL/download/v26.1/protoc-26.1-linux-x86_64.zip
    # Unzip the binary
    unzip "$INS_PATH"/protoc.zip -d "$INS_PATH"

    # Remove stale files
    rm -f "$INS_PATH"/protoc.zip
    rm -f "$INS_PATH"/readme.txt
fi
