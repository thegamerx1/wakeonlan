require("dotenv").config()
const express = require("express")
const wake = require("wakeonlan")
const ping = require("ping")
const { body, validationResult } = require("express-validator")

const app = express()

app.use(express.json())

app.use((req, res, next) => {
    res.setHeader("Access-Control-Allow-Origin", "*")
    res.setHeader("Access-Control-Allow-Methods", "POST,GET,OPTIONS")
    res.setHeader("Access-Control-Allow-Headers", "content-type,xkey")
	res.setHeader("Access-Control-Allow-Credentials","true")
	res.setHeader("Access-Control-Max-Age", "120")
	if ("OPTIONS" == req.method)  return res.sendStatus(200)
	next()
})

app.post("/login",
	(req, res) => {
		res.json({success: req.body.key === process.env.key})
	}
)

app.use((req, res, next) => {
	if (req.headers.xkey === process.env.key) {
		next()
	} else {
		res.status(403).send()
	}
})

app.post("/wake",
	body("mac").notEmpty(),
	(req, res) => {
        const errors = validationResult(req)
        if (!errors.isEmpty()) {
            return res.status(400).json({
                success: false,
                errors: errors.array()
            })
        }
		wake(req.body.mac).then(() => {
			res.json({success: true})
		}).catch((e) => {
			res.json({success: false, error: e})
		})
	}
)

app.post("/ping",
	body("host").notEmpty(),
	(req, res) => {
        const errors = validationResult(req)
        if (!errors.isEmpty()) {
            return res.status(400).json({
                success: false,
                errors: errors.array()
            })
        }

		ping.sys.probe(req.body.host, ping => {
			res.json({success: ping})
		}, {timeout: 1})
	}
)

app.listen(process.env.port || 80, () => {
	console.log("Ready, code: " + process.env.key)
})