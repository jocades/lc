let id = fun x -> x
let a = id 1
let b = id true

let _ =
  let g x = x in
  let _ = g 1 in
  g true

let _ =
  let a f x = f x in
  a id 1
