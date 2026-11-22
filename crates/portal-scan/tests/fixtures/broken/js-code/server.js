const fs = require("fs")
const { execSync } = require("child_process")
const CONFIG = "/Users/ana/app/config.json"
const data = __dirname + "/data"
const out = `${process.cwd()}/out`
const home = process.env.HOME
execSync("rm -rf build")
const port = process.env.PORT || 3000
