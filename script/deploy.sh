#!/bin/bash

cargo build --release

cp target/release/git-exp ~/.local/bin/.
