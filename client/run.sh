#! /bin/bash
#Secure Aggregation Server

for (( counter=$1; counter>0; counter-- ))
do
sleep 0.2 && (./target/debug/client "Client $counter" & echo "Client $counter")
done

#END