CREATE TABLE "metadata" (
	"name"	TEXT NOT NULL UNIQUE,
	"value"	TEXT,
	PRIMARY KEY("name")
);

INSERT INTO "metadata"
("name", "value")
VALUES ('version', '0');

CREATE TABLE "Logs" (
	"id"	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	"check_id"	INTEGER NOT NULL,
	"time"	INTEGER NOT NULL,
	"result_type"	TEXT NOT NULL DEFAULT 'up',
	"result_info"	TEXT DEFAULT NULL
);

CREATE INDEX "log_time_index" ON "Logs" (
	"check_id"	ASC,
	"time"	ASC
);

CREATE TABLE "LogCount" (
	"check_id"	INTEGER NOT NULL,
	"up_to"	INTEGER NOT NULL,
	"count_up"	INTEGER NOT NULL,
	"count_warn"	INTEGER NOT NULL,
	"count_error"	INTEGER NOT NULL
);

CREATE UNIQUE INDEX "log_count_up_to_index" ON "LogCount" (
	"check_id"	ASC,
	"up_to"	ASC
);

CREATE TABLE "pushSubscriptions" (
	"endpoint_url"	TEXT NOT NULL,
	"check_id"	INTEGER NOT NULL,
	"auth"	BLOB NOT NULL,
	"client_p256dh"	INTEGER,
	"notify_warn"	INTEGER NOT NULL
);

CREATE INDEX "push_subs_check_id" ON "pushSubscriptions" (
	"check_id"	ASC
);
