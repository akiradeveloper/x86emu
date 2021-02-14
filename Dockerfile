from 'ubuntu:21.04'
WORKDIR '/x86emu'

RUN apt-get update
RUN apt-get install -y build-essential
RUN apt-get install -y nasm

ARG USER
ARG UID
RUN groupadd ${USER}
RUN useradd -d /home/${USER} -m -s /bin/bash -u ${UID} -g ${USER} -G sudo ${USER}
USER ${USER}