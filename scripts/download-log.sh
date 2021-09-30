#! /bin/sh
server="staging"
scp -r massbit@${server}.massbit.io:massbitprotocol/log ./${server}-log
cd ./${server}-log && gunzip -fdk *.gz
