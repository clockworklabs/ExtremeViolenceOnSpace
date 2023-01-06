#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

echo "Setup DB.."

spacetime publish extremeviolenceonspace --clear-database
sleep 3
spacetime call extremeviolenceonspace init_tournament
sleep 2

echo "Done"