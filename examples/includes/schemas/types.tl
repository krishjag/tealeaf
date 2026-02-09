# Shared schema definitions for the project API context.
# These types are reused across multiple data files.

@struct endpoint (method: string, path: string, description: string, params: []param?, response: string)
@struct param (name: string, type: string, required: bool)
@struct member (name: string, role: string, email: string)
