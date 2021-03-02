#! /bin/bash
#Secure Aggregation Server

for (( counter=$1; counter>0; counter-- ))
do
sleep 0.01 && (./target/debug/client "Client $counter" $2 & echo "Client $counter")
done

#END