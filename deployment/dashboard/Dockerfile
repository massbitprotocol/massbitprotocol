FROM node:14-alpine

RUN apt update
RUN apt install -y npm

RUN git clone https://github.com/massbitprotocol/dashboard
WORKDIR "dashboard"
RUN npm install

CMD npm run serve