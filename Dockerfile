from 'debian:buster'
WORKDIR '/x86emu'

RUN apt-get update
RUN apt-get install -y build-essential
RUN apt-get install -y nasm
RUN apt-get install -y curl

ARG USER
ARG UID
RUN groupadd ${USER}
RUN useradd -d /home/${USER} -m -s /bin/bash -u ${UID} -g ${USER} -G sudo ${USER}
USER ${USER}

RUN curl https://sh.rustup.rs -sSf >> ${HOME}/rustup.rs
RUN sh ${HOME}/rustup.rs -y
RUN echo $HOME
ENV PATH=/home/${USER}/.cargo/bin:$PATH
RUN echo $PATH