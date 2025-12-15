import type React from "react"
import type { Metadata } from "next"
import { Geist_Mono as GeistMono } from "next/font/google"
import "./globals.css"
import Web3Provider from "@/components/shared/Web3Provider"
import { ThemeProvider } from "@/components/theme-provider"
import { Toaster } from "@/components/ui/sonner"

const geistMono = GeistMono({ subsets: ["latin"] })

export const metadata: Metadata = {
  title: "Shadow Swap - Privacy-Enhanced Cross-Chain Bridge",
  description: "Bridge assets across chains privately and instantly with Shadow Swap on Mantle blockchain",
  generator: "v0.app",
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={`${geistMono.className} bg-black text-white antialiased`}>
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem={false}>
          <Web3Provider>
            {children}
            <Toaster />
          </Web3Provider>
        </ThemeProvider>
      </body>
    </html>
  )
}
