#!/bin/bash
echo "Run deploy script"
rsync -avz .bash_profile huy@34.159.83.129:./
rsync -avz .bash_profile huy@34.159.170.173:./
rsync -avz .bash_profile huy@34.89.174.48:./

