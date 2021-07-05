#! /bin/bash

# Not supplying IP and ports
# Default value is localhost 8888 and 9999

# Malicious:
#	./runall.sh [client number] [vector length] [input bit limit] [dropouts] 
#				[session time] [IS session time] [corrupted parties] [malicious flag]
#				[ip] [msg port] [broadcast port2]

# Semi-Honest:
#	./runall.sh [client number] [vector length] [dropouts] 
#				[session time] [IS session time] [malicious flag]
#				[ip] [msg port] [broadcast port2]

if [[ $# == 8 ]]
# malicious without port
	then
		cd server
		cargo run $1 $2 $3 $4 $5 $6 $7 $8 &

		cd ..
		cd client
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (target/debug/client "Client $counter" $2 $8 $3 & echo "Client $counter")
		done
fi
if [[ $# == 6 ]]
# semi-honest without port
	then
		cd server
		cargo run $1 $2 $3 $4 $5 $6 &

		cd ..
		cd client
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (target/debug/client "Client $counter" $2 $6 & echo "Client $counter")
		done
fi

# Supplying IP and ports
# $9 = IP, $10 = port1 for messeging, $11 = port2 for broadcasting 
if [[ $# == 11 ]]
# malicious with port
	then
		cd server
		cargo run $1 $2 $3 $4 $5 $6 $7 $8 $9 ${10} ${11} &

		cd ..
		cd client
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (target/debug/client "Client $counter" $2 $8 $3 $9 ${10} ${11} & echo "Client $counter")
		done
fi
if [[ $# == 9 ]]
# semi-honest with port
	then
		cd server
		cargo run $1 $2 $3 $4 $5 $6 $7 $8 $9 &

		cd ..
		cd client
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (target/debug/client "Client $counter" $2 $6 $7 $8 $9 & echo "Client $counter")
		done
fi
# ./runall.sh 20 64 3 0 20000 20000 0 true
#END