#! /bin/bash

cd server
cargo run $6 $7 $1 $2 $3 $4 $5 &

cd ..
cd client
for (( counter=0; counter< $6; counter++ ))
do
sleep 0.01 && (target/debug/client "Client $counter" $7 & echo "Client $counter")
done

#END
