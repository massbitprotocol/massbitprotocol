#!/bin/sh
cd code-compiler
python3 app.py 2>&1 | tee ../log/console-code-compiler.log
