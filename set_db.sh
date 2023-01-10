#!/bin/bash
cd "$(dirname "$0")"

mv Cargo.toml _Cargo.toml

echo "Setup DB.."

cd Server
spacetime publish extremeviolenceonspace --clear-database
sleep 3

cd "$(dirname "$0")"

mv _Cargo.toml Cargo.toml

echo "Done"