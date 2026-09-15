import React, {useState} from 'react';
// Demonstrates ordinary third-party-style hooks and a value callback to Rust.
export function ClientCounter({value, onValueChange, ...props}) {
  const [clicks,setClicks]=useState(0);
  return React.createElement('button',{...props,type:'button',onClick:()=>{
    setClicks(n=>n+1);onValueChange?.(value+1);
  }},`Rust: ${value}; local clicks: ${clicks}`);
}
