# docker-compose up -d --build

# docker-compose -f docker-cluster.yml up -d --build 
# docker-compose -f docker-bank.yml up -d --build

docker-compose ps

docker-compose exec connect kafka-topics --create --if-not-exists --zookeeper zookeeper:2181 --partitions 1 --replication-factor 1 --topic account_creation_confirmed
docker-compose exec connect kafka-topics --create --if-not-exists --zookeeper zookeeper:2181 --partitions 1 --replication-factor 1 --topic account_creation_failed
docker-compose exec connect kafka-topics --create --if-not-exists --zookeeper zookeeper:2181 --partitions 1 --replication-factor 1 --topic money_transfer_confirmed
docker-compose exec connect kafka-topics --create --if-not-exists --zookeeper zookeeper:2181 --partitions 1 --replication-factor 1 --topic money_transfer_failed
docker-compose exec connect kafka-topics --create --if-not-exists --zookeeper zookeeper:2181 --partitions 1 --replication-factor 1 --topic confirm_account_creation
docker-compose exec connect kafka-topics --create --if-not-exists --zookeeper zookeeper:2181 --partitions 1 --replication-factor 1 --topic confirm_money_transfer
docker-compose exec connect kafka-topics --create --if-not-exists --zookeeper zookeeper:2181 --partitions 1 --replication-factor 1 --topic balance_changed