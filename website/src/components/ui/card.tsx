import * as React from "react"
import { cn } from "../../lib/utils"
export function Card({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) { return <div className={cn("flex flex-col gap-6 rounded-2xl border bg-card py-6 text-card-foreground", className)} {...props} /> }
export function CardHeader({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) { return <div className={cn("grid auto-rows-min gap-2 px-6", className)} {...props} /> }
export function CardTitle({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) { return <div className={cn("font-semibold leading-none", className)} {...props} /> }
export function CardDescription({ className, ...props }: React.HTMLAttributes<HTMLParagraphElement>) { return <p className={cn("text-sm leading-6 text-muted-foreground", className)} {...props} /> }
export function CardContent({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) { return <div className={cn("px-6", className)} {...props} /> }
export function CardFooter({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) { return <div className={cn("flex items-center px-6", className)} {...props} /> }
