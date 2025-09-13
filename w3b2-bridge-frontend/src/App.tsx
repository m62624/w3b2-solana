import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { WalletProvider } from './contexts/WalletContext';
import { ApiProvider } from './contexts/ApiContext';
import { WebSocketProvider } from './contexts/WebSocketContext';
import Layout from './components/Layout';
import Dashboard from './components/Dashboard';
import Wallet from './components/Wallet';
import Funding from './components/Funding';
import Records from './components/Records';
import Settings from './components/Settings';
import Examples from './components/Examples';
import WalletManagement from './components/WalletManagement';
import './App.css';

function App() {
  return (
    <WalletProvider>
      <ApiProvider>
        <WebSocketProvider>
          <Router>
            <div className="App">
              <Layout>
                <Routes>
                  <Route path="/" element={<Dashboard />} />
                  <Route path="/wallet" element={<Wallet />} />
                  <Route path="/funding" element={<Funding />} />
                  <Route path="/records" element={<Records />} />
                  <Route path="/settings" element={<Settings />} />
                  <Route path="/examples" element={<Examples />} />
                  <Route path="/wallet-management" element={<WalletManagement />} />
                </Routes>
              </Layout>
              <Toaster
                position="top-right"
                toastOptions={{
                  duration: 4000,
                  style: {
                    background: '#363636',
                    color: '#fff',
                  },
                  success: {
                    duration: 3000,
                    iconTheme: {
                      primary: '#4ade80',
                      secondary: '#fff',
                    },
                  },
                  error: {
                    duration: 5000,
                    iconTheme: {
                      primary: '#ef4444',
                      secondary: '#fff',
                    },
                  },
                }}
              />
            </div>
          </Router>
        </WebSocketProvider>
      </ApiProvider>
    </WalletProvider>
  );
}

export default App;