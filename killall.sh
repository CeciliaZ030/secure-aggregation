#! /bin/bash

sudo kill -9 $(lsof -ti:5555) 
sudo kill -9 $(lsof -ti:6666)
pkill client 
