import express from 'express'
import compression from 'compression'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const app = express()
const port = process.env.PORT || 3000

app.use(compression())
app.use(express.static(path.join(here, '../dist')))
app.get('/api/catalog', async (_req, res) => {
  const upstream = await fetch(`${process.env.CATALOG_URL}/items`)
  res.json(await upstream.json())
})

app.listen(port, () => console.log(`shop-web on http://localhost:${port}`))
