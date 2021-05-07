#! /bin/bash

# Not supplying IP and ports
# Default value is localhost 8888 and 9999
if [[ $# == 8 ]]
	then
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (client/target/debug/client "Client $counter" $2 $3 & echo "Client $counter")
		done
fi
# Supplying IP and ports
# $9 = IP, $10 = port1 for messeging, $11 = port2 for broadcasting 
if [[ $# == 11 ]]
	then
		for (( counter=0; counter< $1; counter++ ))
			do
			sleep 0.01 && (client/target/debug/client "Client $counter" $2 $3 $9 ${10} ${11} & echo "Client $counter")
		done
fi

# ./runall.sh 20 64 3 0 20000 20000 0 true
#END
