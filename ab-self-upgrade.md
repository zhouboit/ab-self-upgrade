# A/B Self Upgrade Lib

## PRD
### 1、current program is a a/b self upgrade client
### 2、cloud provider a version metadata for json file contains latest version for each supported arch installer pkg download uri and checksum
### 3、the client compare current version is older than metadata then download new version installer and verify checksum, version str like `0.1.2` `0.1.3` `1.0.0` 
### 4、download ok is implement a/b upgrade, download uri=${api_domain}${self_upgrade_metadata}
### 5、upgrade success or failed report state to cloud, and when upgrade failed keep older running

## MUST
### on the upgrade process the older program is always running before new version start success and report heartbeat ok
### configuration
```json
api_domain = https://cloud.example.com
auto_upgrade_enabled = true 
auto_upgrade_metadata = /xxx/ccc/meta.json
# minutes
auto_upgrade_check_frequency = 10
```