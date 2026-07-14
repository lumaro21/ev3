import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.jsx'
import { RobotProvider } from './context/RobotContext.jsx' 

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <RobotProvider>
      <App />
    </RobotProvider>
  </React.StrictMode>,
)