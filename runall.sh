#! /bin/bash

if [[ $# == 8 ]]
	then
		cd server
		cargo run $1 $2 $3 $4 $5 $6 $7 $8 &

		cd ..
		cd client
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (target/debug/client "Client $counter" $2 $3 & echo "Client $counter")
		done
fi

if [[ $# == 11 ]]
	then
		cd server
		cargo run $1 $2 $3 $4 $5 $6 $7 $8 $9 ${10} ${11} &

		cd ..
		cd client
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (target/debug/client "Client $counter" $2 $3 $9 ${10} ${11} & echo "Client $counter")
		done
fi

# ./runall.sh 20 64 3 0 20000 20000 0 true
#END