import sqlite3, os, random
p = os.path.join(os.path.dirname(os.path.abspath(__file__)), "shop.db")
if os.path.exists(p): os.remove(p)
c = sqlite3.connect(p)
c.executescript("""
PRAGMA foreign_keys = ON;
CREATE TABLE categories (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  parent_id INTEGER REFERENCES categories(id)
);
CREATE TABLE products (
  id INTEGER PRIMARY KEY,
  category_id INTEGER NOT NULL REFERENCES categories(id),
  sku TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  price REAL NOT NULL
);
CREATE TABLE customers (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  city TEXT,
  created_at TEXT NOT NULL
);
CREATE TABLE orders (
  id INTEGER PRIMARY KEY,
  customer_id INTEGER NOT NULL REFERENCES customers(id),
  status TEXT NOT NULL,
  ordered_at TEXT NOT NULL
);
CREATE TABLE order_items (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id),
  product_id INTEGER NOT NULL REFERENCES products(id),
  qty INTEGER NOT NULL,
  unit_price REAL NOT NULL
);
CREATE TABLE payments (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id),
  method TEXT NOT NULL,
  amount REAL NOT NULL,
  paid_at TEXT
);
CREATE TABLE shipments (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL UNIQUE REFERENCES orders(id),
  carrier TEXT NOT NULL,
  shipped_at TEXT
);
""")
cats = [(1,"Kitchen",None),(2,"Garden",None),(3,"Cookware",1),(4,"Tools",2)]
c.executemany("INSERT INTO categories VALUES (?,?,?)", cats)
prods = [(1,3,"KT-100","Cast Iron Pan",34.0),(2,3,"KT-101","Saucepan",22.5),(3,1,"KT-200","Bamboo Board",12.0),
         (4,4,"GD-300","Hand Trowel",9.5),(5,4,"GD-301","Pruning Shears",18.0),(6,2,"GD-400","Seed Tray",6.0)]
c.executemany("INSERT INTO products VALUES (?,?,?,?,?)", prods)
names = ["Customer A","Customer B","Customer C","Customer D","Customer E","Customer F"]
cities = ["Northvale","Eastport","Lakeside","Riverton","Hillcrest","Westbrook"]
c.executemany("INSERT INTO customers VALUES (?,?,?,?)", [(i+1,n,cities[i],f"2026-0{1+i%6}-1{i}") for i,n in enumerate(names)])
random.seed(7)
oid=0; iid=0
for o in range(1,13):
    c.execute("INSERT INTO orders VALUES (?,?,?,?)",(o,1+o%6,random.choice(["paid","shipped","delivered"]),f"2026-10-{o:02d}"))
    tot=0
    for k in range(random.randint(1,3)):
        iid+=1; pr=random.choice(prods); q=random.randint(1,3); tot+=q*pr[4]
        c.execute("INSERT INTO order_items VALUES (?,?,?,?,?)",(iid,o,pr[0],q,pr[4]))
    c.execute("INSERT INTO payments VALUES (?,?,?,?,?)",(o,o,random.choice(["card","transfer"]),round(tot,2),f"2026-10-{o:02d}"))
    if o<=9: c.execute("INSERT INTO shipments VALUES (?,?,?,?)",(o,o,random.choice(["Carrier One","Carrier Two"]),f"2026-10-{o+1:02d}"))
c.commit()
print(c.execute("select name from sqlite_master where type='table'").fetchall())
print(c.execute("PRAGMA foreign_key_check").fetchall())
