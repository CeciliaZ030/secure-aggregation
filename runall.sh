#! /bin/bash

cd server
cargo run $1 $2 $3 $4 $5 $6 $7 $8&

cd ..
cd client
for (( counter=0; counter< $1; counter++ ))
do
sleep 0.01 && (target/debug/client "Client $counter" $2 $3 & echo "Client $counter")
done

# ./runall.sh 20 64 3 0 20000 20000 0 true
#END