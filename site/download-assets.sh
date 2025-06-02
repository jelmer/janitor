#!/bin/bash
set -e

# Script to download third-party JavaScript libraries for the Janitor site
# This ensures we have all dependencies bundled rather than relying on system packages

STATIC_DIR="$(dirname "$0")/static"
JS_DIR="$STATIC_DIR/js"
IMG_DIR="$STATIC_DIR/img"

echo "Downloading third-party JavaScript libraries..."

# Create directories
mkdir -p "$JS_DIR" "$IMG_DIR/datatables"

# Download jQuery
echo "Downloading jQuery..."
curl -o "$JS_DIR/jquery.js" "https://code.jquery.com/jquery-3.7.1.min.js"

# Download DataTables
echo "Downloading DataTables..."
curl -o "$JS_DIR/jquery.datatables.js" "https://cdn.datatables.net/1.13.7/js/jquery.dataTables.min.js"

# Download DataTables images
echo "Downloading DataTables images..."
curl -o "$IMG_DIR/datatables/sort_asc.png" "https://cdn.datatables.net/1.13.7/images/sort_asc.png"
curl -o "$IMG_DIR/datatables/sort_desc.png" "https://cdn.datatables.net/1.13.7/images/sort_desc.png"
curl -o "$IMG_DIR/datatables/sort_both.png" "https://cdn.datatables.net/1.13.7/images/sort_both.png"

# Download jQuery Typeahead
echo "Downloading jQuery Typeahead..."
curl -o "$JS_DIR/jquery.typeahead.js" "https://cdnjs.cloudflare.com/ajax/libs/jquery-typeahead/2.11.2/jquery.typeahead.min.js"

# Download Moment.js
echo "Downloading Moment.js..."
curl -o "$JS_DIR/moment.js" "https://cdnjs.cloudflare.com/ajax/libs/moment.js/2.30.1/moment.min.js"

# Download Chart.js
echo "Downloading Chart.js..."
curl -o "$JS_DIR/chart.js" "https://cdnjs.cloudflare.com/ajax/libs/Chart.js/4.4.0/chart.min.js"

echo "Downloaded all third-party JavaScript libraries to $JS_DIR"
echo "Downloaded DataTables images to $IMG_DIR/datatables"
echo ""
echo "Asset structure:"
echo "static/"
echo "├── css/ (copied from Python implementation)"
echo "├── js/ (downloaded dependencies + janitor.js)"
echo "└── img/ (copied images + datatables icons)"