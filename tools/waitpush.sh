#!/bin/bash
# Exit (printing the new sha) when the remote work branch moves past $1.
# usage: waitpush.sh <known-sha>
cd /home/user/flowmaid
known=$1
while true; do
  cur=$(git ls-remote origin refs/heads/ccr-ccc6d89a-5cxd7e 2>/dev/null | cut -c1-40)
  if [ -n "$cur" ] && [ "${cur:0:7}" != "${known:0:7}" ]; then echo "PUSHED $cur"; exit 0; fi
  sleep 60
done
