FROM rustlang/rust:nightly-buster

RUN apt update && apt install -y python3-pip
RUN mkdir /app
COPY ./code-compiler/requirements.txt /tmp/requirements.txt
RUN pip3 install -r /tmp/requirements.txt
RUN rustup target add wasm32-unknown-unknown

COPY . /app
WORKDIR /app/code-compiler

EXPOSE 5000

CMD [ "python3", "-m", "flask", "run", "--host=0.0.0.0"]
