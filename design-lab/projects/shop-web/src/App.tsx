import { useEffect, useState } from 'react'

type Item = { id: string; name: string; price: number }

export function App() {
  const [items, setItems] = useState<Item[]>([])
  useEffect(() => {
    fetch('/api/catalog').then((r) => r.json()).then(setItems)
  }, [])
  return (
    <ul>
      {items.map((i) => (
        <li key={i.id}>{i.name} · {i.price}</li>
      ))}
    </ul>
  )
}
