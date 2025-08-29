-- Create the accounts table
CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL, -- e.g., 'Float Assets', 'Customer Escrow Payable', 'Merchant Payable', 'Clearing Receivable/Payable', 'Fee Revenue'
    currency TEXT NOT NULL, -- ISO 4217 code, e.g., 'XAF'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create the journals table
CREATE TABLE journals (
    id TEXT PRIMARY KEY NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create the journal_entries (legs) table
CREATE TABLE journal_entries (
    id TEXT PRIMARY KEY NOT NULL,
    journal_id TEXT NOT NULL REFERENCES journals(id),
    account_id TEXT NOT NULL REFERENCES accounts(id),
    entry_type TEXT NOT NULL, -- 'DEBIT' or 'CREDIT'
    amount_minor_units BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_amount_positive CHECK (amount_minor_units >= 0),
    CONSTRAINT chk_entry_type CHECK (entry_type IN ('DEBIT', 'CREDIT'))
);

-- Indexes for efficient querying
CREATE INDEX idx_journal_entries_journal_id ON journal_entries (journal_id);
CREATE INDEX idx_journal_entries_account_id ON journal_entries (account_id);
CREATE INDEX idx_accounts_type ON accounts (type);

-- Trigger to update `updated_at` column automatically
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_accounts_updated_at
BEFORE UPDATE ON accounts
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_journals_updated_at
BEFORE UPDATE ON journals
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_journal_entries_updated_at
BEFORE UPDATE ON journal_entries
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();