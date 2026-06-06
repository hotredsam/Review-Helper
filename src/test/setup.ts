import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// Unmount rendered components after each test so renders don't stack across tests.
afterEach(cleanup);
