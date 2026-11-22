const path = require("path")
const express = require("express")
const app = express()
app.use(express.static(path.join(__dirname, "public")))
app.listen(process.env.PORT || 3000)
app.get("/api/items", async (_req, res) => {
  const upstream = await fetch(`${process.env.CATALOG_URL}/items`)
  const base = process.env.API_BASE || "https://api.example.org"
  const user = await fetch(`${base}/users/${_req.query.id}`)
  res.json({ items: await upstream.json(), user: await user.json() })
})
