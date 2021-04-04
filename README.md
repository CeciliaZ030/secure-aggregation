# secure-aggregation

## How to run:

	- Server
		○ $ cargo run [client_num] [Vector_len] [dropouts] [corrupted_num] [malicious_or_not]
	- Client
		○ $./run.sh  [client_num] [Vector_len]
		○ If you make changes to client code, before running bash script do $ cargo build

Also you need to kill all the ports and threads after finish, otherwise it will keep taking over the ports and you cannot run it again.
	- To kill both program:
		○ $ ./killall.sh
		○ Killall.sh is below
    
      #! /bin/bash
      kill $(lsof -ti:9999)
      kill $(lsof -ti:8888)
      pkill client
