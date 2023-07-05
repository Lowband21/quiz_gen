#!/bin/bash

# Array of URLs
urls=("https://cs.du.edu/~ftl/videos1353/" "https://cs.du.edu/~ftl/videos1351/")

# Loop through all URLs
for base_url in "${urls[@]}"; do
    # Get the webpage and extract the file URLs
    wget -qO- "$base_url" | grep -o 'href="[^"]*"' | cut -d'"' -f2 | while read -r line 
    do
        # Append the base URL to each file
        file_url="$base_url$line"

        # Download each file
        wget "$file_url"
    done
done
