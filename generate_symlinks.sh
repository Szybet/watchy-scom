#!/bin/bash
cd ws-serial-tcp/src
ln -P ../../serial.rs serial.rs
ln -P ../../api.rs api.rs
cd ../../
cd watchy-scom/src
ln -P ../../serial.rs serial.rs
ln -P ../../api.rs api.rs
cd ../../