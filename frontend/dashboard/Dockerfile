# Docker file for building code-compiler
FROM ubuntu:18.04

RUN apt update
RUN apt install -y git curl
RUN apt install -y npm
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash -
RUN apt-get install -y nodejs

RUN git clone https://github.com/massbitprotocol/dashboard
WORKDIR "dashboard"
RUN npm install

CMD npm run serve