#! /bin/bash

kill $(lsof -ti:9999) 
kill $(lsof -ti:8888)
pkill client 
