require("dotenv").config()
const express = require("express")
const { body, validationResult } = require("express-validator")
const routes = require("./routes")

const app = express()
app.use(express.json())

app.use((req, res, next) => {
	res.setHeader("Access-Control-Allow-Origin", "*")
	res.setHeader("Access-Control-Allow-Methods", "POST,GET,OPTIONS")
	res.setHeader("Access-Control-Allow-Headers", "*")
	res.setHeader("Access-Control-Allow-Credentials", "true")
	res.setHeader("Access-Control-Max-Age", "120")
	if ("OPTIONS" == req.method) return res.sendStatus(200)
	next()
})

app.post("/login", routes.login)

app.use((req, res, next) => {
	if (req.headers.key === process.env.key) {
		next()
	} else {
		res.status(403).send()
	}
})

app.post("/save", body("devices").isArray(), validate(routes.save))
app.post("/wake", body("mac").notEmpty(), validate(routes.wake))
app.post("/ping", body("hosts").isArray(), validate(routes.ping))

function validate(call) {
	return (req, res) => {
		const errors = validationResult(req)
		if (!errors.isEmpty()) {
			return res.status(400).json({
				success: false,
				errors: errors.array(),
			})
		}
		call(req, res)
	}
}

const port = process.env.port || 80
app.listen(port, () => {
	console.log(`Listening at port ${port} code: ${process.env.key}`)
})
