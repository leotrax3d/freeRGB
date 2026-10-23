import { expect, it } from "vitest";
import { installationProgress } from "./InstallProgress";

it("never announces completion before the service confirms it",()=>{
  expect(installationProgress(10000,100,false)).toBeLessThan(100);
  expect(installationProgress(20,100,true)).toBeLessThan(100);
  expect(installationProgress(900,100,true)).toBe(100);
  expect(installationProgress(200,80,false)).toBe(80);
});
