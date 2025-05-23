#!/bin/bash

# Define source directory
SRC_DIR="src"
# Name of the executable
EXECUTABLE="main"

# Ensure the source directory exists
if [ ! -d "$SRC_DIR" ]; then
  echo "Error: Source directory '$SRC_DIR' not found."
  exit 1
fi

# Compile Swift source files
swiftc "$SRC_DIR"/main.swift "$SRC_DIR"/ls.swift "$SRC_DIR"/cat.swift "$SRC_DIR"/color.swift "$SRC_DIR"/cd.swift "$SRC_DIR"/sleep.swift "$SRC_DIR"/time.swift "$SRC_DIR"/cp.swift "$SRC_DIR"/mkdir.swift "$SRC_DIR"/rm.swift -o "$EXECUTABLE"

# Check if compilation was successful
if [ $? -eq 0 ]; then
  echo "Compilation successful."
else
  echo "Compilation failed."
fi
