#! /bin/bash

cd server
cargo run $1 $2 $3 $4 $5 $6 &

cd ..
cd client
for (( counter=0; counter< $1; counter++ ))
do
sleep 0.01 && (target/debug/client "Client $counter" $2 & echo "Client $counter")
done

#END