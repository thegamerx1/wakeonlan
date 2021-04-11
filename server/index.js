require("dotenv").config()
const express = require("express")
const wake = require("wakeonlan")
const ping = require("ping")
const { body, validationResult } = require("express-validator")

const app = express()

app.use(express.json())

app.use((req, res, next) => {
    res.setHeader("Access-Control-Allow-Origin", "*")
    res.setHeader("Access-Control-Allow-Methods", "POST")
    res.setHeader("Access-Control-Allow-Headers", "content-type,xkey")
    next()
})

app.options("*", (req, res) => {
	res.status(204).send("")
})

app.post("/login",
	body("key").isMACAddress(),
	(req, res) => {
		res.status(req.body.key === process.env.key ? 200 : 403).send()
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
	body("mac").isMACAddress(),
	(req, res) => {
        const errors = validationResult(req)
        if (!errors.isEmpty()) {
            return res.status(400).json({
                success: false,
                errors: errors.array()
            })
        }
		wake(req.body.mac).then(() => {
			res.status(204).send()
		}).catch((e) => {
			res.status(400).send()
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
			res.status(ping ? 204 : 404).send("")
		}, {timeout: 1})
	}
)

app.listen(80, () => {
	console.log("Ready, code: " + process.env.key)
})