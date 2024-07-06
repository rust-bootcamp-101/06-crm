# 从数据库导出数据100条
# 先在pg里面将数据导到一张表里
# create table export_user_stats as select * from user_stats limit 100;

pg_dump:
	@pg_dump --table=export_user_stats --data-only --column-inserts stats > ./user-stat/fixtures/test.sql
	@sed 's/public.export_user_stats/user_stats/' ./user-stat/fixtures/test.sql > ./user-stat/fixtures/temp.sql && mv ./user-stat/fixtures/temp.sql ./user-stat/fixtures/test.sql


crm_tests:
	@cd ./user-stat && cargo run
	@cd ./crm-metadata && cargo run
	@cd ./crm-send && cargo run
