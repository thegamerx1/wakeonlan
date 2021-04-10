require("dotenv").config()
const express = require("express")
const wake = require("wakeonlan")
const ping = require("ping")
const { body, validationResult } = require("express-validator")

const app = express()
app.use((req, res, next) => {
	if (req.headers.xkey === process.env.key) {
		next()
	} else {
		res.status(403).send()
	}
})

app.get("/wake",
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
			res.status(400).send(e)
		})
	}
)

app.get("/wake",
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
			res.status(400).send(e)
		})
	}
)

app.get("/ping",
	body("host").matches(/^[\w\-\d]$/i),
	(req, res) => {
        const errors = validationResult(req)
        if (!errors.isEmpty()) {
            return res.status(400).json({
                success: false,
                errors: errors.array()
            })
        }

		ping.sys.probe(req.body.host, ping => {
			res.send(ping)
			console.log(ping)
		}, {timeout: 2})
	}
)

app.listen(80, () => {
	console.log("Ready, code: " + process.env.key)
})