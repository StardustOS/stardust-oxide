#!/usr/bin/env bash

# Helper script for `cargo run` command

# print section sizes
size $1

# print total size
dua $1

# create temporary file for config
temp_file=$(mktemp)

# delete temporary file after signal (0: exit shell, 2: interrupt, 3: quit, 15: terminate)
trap "rm -f $temp_file" 0 2 3 15

# create config fuke
echo "kernel = \"$1\"" >> $temp_file
echo "memory = 4" >> $temp_file
echo "name = \"stardust\"" >> $temp_file
echo "on_crash = 'destroy'" >> $temp_file

# start VM
sudo xl create -c $temp_file
