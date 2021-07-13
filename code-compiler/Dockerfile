# Docker file for building code-compiler
FROM ubuntu:18.04

RUN apt update
RUN apt install -y git

COPY ./ massbitprotocol
WORKDIR "massbitprotocol/code-compiler" 

RUN ls
RUN apt install -y python3
RUN apt install -y python3-pip
RUN pip3 install -U Flask
RUN pip3 install -U flask-cors
RUN pip3 install -U ipfshttpclient
CMD python3 app.py