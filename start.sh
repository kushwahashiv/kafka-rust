docker-compose -f docker-cluster.yml up -d --build 
docker-compose -f docker-bank.yml up -d --build

docker-compose ps

 confluent platform @ http://localhost:9021/

topics:

 account_creation_confirmed
 account_creation_failed
 money_transfer_confirmed
 money_transfer_failed
 confirm_account_creation
 confirm_money_transfer
 balance_changed 
