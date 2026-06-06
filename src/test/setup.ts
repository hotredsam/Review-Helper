import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

// Unmount rendered components after each test so renders don't stack across tests.
afterEach(cleanup);
