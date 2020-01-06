# http PUT http://127.0.0.1:8083/connectors < connector/jdbc-source.json
# http PUT http://127.0.0.1:8083/connectors < connector/jdbc-sink.json

docker-compose exec connect-debezium bash -c '/connector/create-pg-source.sh'
docker-compose exec kafka-connect-cp bash -c '/connector/create-pg-sink.sh'