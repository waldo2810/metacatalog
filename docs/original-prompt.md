what would it take to build a data lineage, cataloging tool. At my current company, we've been facing this problem where the client does not want to pay for Collate or Atlan or those tools, too expensive and overkill for them.

And honestly, there's just 1 use case they care about right now, data lineage. So there's a huge Excel spreadsheet with 50 tabs each containing mappings, usages by processes, XMATCHs XLOOKUPs between each others, and collaborating, or having visibility's not very good. It might work with 1 source, but there are 2 sources of information, and things might get tricky with another source or upstream/downstream processes.

I'm aware of OpenMetadata and UnityCatalog, and I love those, but I'd like to have my own implementation for now. In terms of having "connectors" to extract schemas from azure vm's sql server's adding datawarehouse fields, then data marts. Is this something viable? How much effort (in months) would it take?
