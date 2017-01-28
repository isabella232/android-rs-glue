FROM ubuntu:xenial

RUN apt-get update
RUN apt-get install -yq sudo curl wget git file g++ cmake pkg-config \
                        libasound2-dev bison flex unzip ant openjdk-8-jdk \
                        lib32stdc++6 lib32z1

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH /root/.cargo/bin:$PATH
RUN rustup default nightly
RUN rustup target add arm-linux-androideabi

RUN mkdir /root/cargo-apk
COPY . /root/cargo-apk
RUN cargo install --path /root/cargo-apk/cargo-apk
RUN rm -rf /root/cargo-apk

ENV ANDROID_HOME /usr/local/android-sdk-linux
RUN cd /usr/local && \
    wget -q https://dl.google.com/android/android-sdk_r24.4.1-linux.tgz && \
    tar -xzf android-sdk_r24.4.1-linux.tgz && \
    rm android-sdk_r24.4.1-linux.tgz
RUN echo y | ${ANDROID_HOME}/tools/android update sdk --no-ui --all --filter platform-tools,android-18,build-tools-23.0.3
ENV PATH $PATH:${ANDROID_HOME}/tools:$ANDROID_HOME/platform-tools

RUN cd /usr/local && \
    wget -q http://dl.google.com/android/repository/android-ndk-r12b-linux-x86_64.zip && \
    unzip -q android-ndk-r12b-linux-x86_64.zip && \
    rm android-ndk-r12b-linux-x86_64.zip

ENV NDK_HOME /usr/local/android-ndk-r12b

RUN mkdir /root/src
WORKDIR /root/src
